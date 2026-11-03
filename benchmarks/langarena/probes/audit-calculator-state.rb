# Token-stream arithmetic oracle; does not call eval or change the canonical port.
require 'json'
require 'digest'
require 'fileutils'
require 'open3'
raise 'usage: FROZEN_KUJO PINNED_UPSTREAM NEW_OUTPUT [wrapping-candidate]' unless ARGV.size == 3 || (ARGV.size == 4 && ARGV[3] == 'wrapping-candidate')
wrapping_candidate = ARGV.size == 4
binary,upstream,out=ARGV.first(3).map { |p| File.expand_path(p) }
raise 'output exists' if File.exist?(out)
FileUtils.mkdir_p(out)
pin='3c9f84ddaeec834c0631747cb789e8cfd602f993'
sources=%w[benchmarks/langarena/src/00_common.kujo benchmarks/langarena/src/70_calculator.kujo]
files=([binary,File.expand_path(__FILE__)]+sources).to_h { |p| [p,Digest::SHA256.file(p).hexdigest] }
%w[rust/src/benchmarks/calculator_ast.rs rust/src/benchmarks/calculator_interpreter.rs goose/src/calculator_ast.goose goose/src/calculator_interpreter.goose].each do |path|
  bytes,err,status=Open3.capture3('git','-C',upstream,'show',"#{pin}:#{path}")
  raise err unless status.success? && bytes==File.binread(upstream+'/'+path)
  files[upstream+'/'+path]=Digest::SHA256.hexdigest(bytes)
  File.write(out+'/'+File.basename(path),bytes)
end
def signed(value)
  ((value+(1<<63))%(1<<64))-(1<<63)
end
class TokenParser
  def initialize(text)
    @tokens=text.scan(/[A-Za-z][A-Za-z0-9]*|[0-9]+|[=()+*\/%-]/)
    @index=0
  end
  def peek; @tokens[@index]; end
  def take; token=peek;@index+=1;token;end
  def expression(minimum=0)
    token=take
    left=if token=='('
      value=expression;raise 'closing parenthesis' unless take==')';value
    elsif token&.match?(/\A[0-9]+\z/)
      ['number',signed(token.to_i)]
    elsif token&.match?(/\A[A-Za-z]/)
      if peek=='='
        take;['assignment',token,expression]
      else
        ['variable',token]
      end
    else
      raise "unexpected token #{token}"
    end
    precedence={'+'=>1,'-'=>1,'*'=>2,'/'=>2,'%'=>2}
    while precedence.fetch(peek,-1)>=minimum
      operator=take
      left=['binary',operator,left,expression(precedence[operator]+1)]
    end
    left
  end
  def program
    nodes=[];nodes<<expression while peek;nodes
  end
end
def evaluate(node,variables)
  case node[0]
  when 'number' then node[1]
  when 'variable' then variables.fetch(node[1],0)
  when 'assignment' then variables[node[1]]=evaluate(node[2],variables)
  when 'binary'
    a=evaluate(node[2],variables);b=evaluate(node[3],variables)
    value=case node[1]
    when '+' then a+b
    when '-' then a-b
    when '*' then a*b
    else
      quotient=b==0 ? 0 : (a.abs/b.abs)*((a<0) != (b<0) ? -1 : 1)
      b==0 ? 0 : node[1]=='/' ? quotient : a-quotient*b
    end
    signed(value)
  end
end
programs=["v0 = 1\n", "a=1+2*3\nb=(a-2)*4\n", "a=20/3\nb=20%3\n", "a=0-20\nb=a/3\nc=a%3\n",
 "a=20\nb=0-3\nc=a/b\nd=a%b\n", "a=1/0\nb=8%0\n", "a=missing+7\n", "a=1\na=a+2\nb=a*3\n",
 "a=9223372036854775807+1\nb=a-1\n", "a=18446744073709551615\nb=a*7\n", "a=3037000500*3037000500\n",
 "a=(b=3)+(b=b+4)\nc=a+b\n", " 	A1 = (((3+4)))\r\nlast = A1*2-1\n", "a=9-3-2\nb=48/4/3\nc=17%5*2\n"]
cases=%w[Calculator::Ast Calculator::Interpreter].flat_map { |name| programs.map { |text| {'name'=>name,'text'=>text} } }
expected=cases.each_with_index.flat_map do |entry,index|
  ast=TokenParser.new(entry['text']).program
  variables={};evaluations=ast.map { |node| value=evaluate(node,variables);{'value'=>value,'variables'=>variables.dup} }
  name_hash=ast.last[1].bytes.reduce(5381) { |hash,byte| (hash*33+byte)&0xffffffff }
  delta=entry['name']=='Calculator::Ast' ? ast.size+name_hash : evaluations.last['value']
  3.times.map { |step| {'case'=>index,'step'=>step,'name'=>entry['name'],'text'=>entry['text'],'ast'=>ast,'evaluations'=>evaluations,'result'=>(4294967290+(step+1)*delta)&0xffffffff} }
end
port_source=sources.map { |p| File.read(p) }.join("\n")
if wrapping_candidate
  old='state["result"] = u32(state["result"] + value)'
  raise 'candidate target' unless port_source.scan(old).size == 1
  port_source=port_source.sub(old,'state["result"] = u32(wrapping_add(state["result"], value))')
end
fixture=port_source+"\nlet cases := parse_json(read_file(#{JSON.generate(out+'/cases.json')}))\n"+<<'KUJO'
func audit_node(node) {
    if node.kind == "CalcNumber" { return ["number", node.value] }
    if node.kind == "CalcVariable" { return ["variable", node.name] }
    if node.kind == "CalcAssignment" { return ["assignment", node.name, audit_node(node.expression)] }
    return ["binary", node.op, audit_node(node.left), audit_node(node.right)]
}
mut index := 0
for entry in cases {
    mut state := basic_state({"name":entry["name"],"operations":0})
    state["text"] = entry["text"]
    state["result"] = 4294967290
    if entry["name"] == "Calculator::Interpreter" { state["ast"] = calc_parse(state["text"]) }
    mut step := 0
    while step < 3 {
        state = calc_run(state, step)
        mut ast := []
        mut evaluations := []
        mut variables := {}
        let empty_snapshot := variables
        for node in state["ast"] {
            ast = push(ast, audit_node(node))
            let evaluated := calc_evaluate(node, variables)
            variables = evaluated[1]
            evaluations = push(evaluations, {"value":evaluated[0],"variables":variables})
        }
        assert_equal(empty_snapshot, {})
        print(to_json({"case":index,"step":step,"name":entry["name"],"text":state["text"],"ast":ast,"evaluations":evaluations,"result":state["result"]}))
        step += 1
    }
    index += 1
}
KUJO
{'fixture.kujo'=>fixture,'cases.json'=>JSON.generate(cases),'expected.json'=>JSON.generate(expected)}.each do |name,bytes|
  File.write(out+'/'+name,bytes);files[out+'/'+name]=Digest::SHA256.hexdigest(bytes)
end
manifest={pin:pin,files:files,cases:cases.size,states_per_runtime:expected.size,wrapping_candidate:wrapping_candidate,commands:[],scope:'Fixed valid programs: full AST, evaluation sequence, variable maps and wrapper result; random preparation, malformed inputs and executed competitors excluded'}
%w[vm interpreter jit].each do |mode|
  files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
  argv=[binary,'run','--scheduler-timeout-ms','120000'];argv<<"--#{mode}" unless mode=='vm';argv<<out+'/fixture.kujo'
  stdout,stderr,status=Open3.capture3(*argv)
  File.write(out+'/'+mode+'.out',stdout);File.write(out+'/'+mode+'.err',stderr)
  manifest[:commands]<<{mode:mode,argv:argv,exit:status.exitstatus,signal:status.termsig}
  File.write(out+'/manifest.json',JSON.pretty_generate(manifest))
  raise "#{mode} failed: #{stderr}" unless status.success?
  actual=stdout.lines.map { |line| JSON.parse(line) }
  mismatch=actual.each_index.find { |i| actual[i]!=expected[i] }
  raise "#{mode} state mismatch #{mismatch}: #{actual[mismatch]} expected #{expected[mismatch]}" if mismatch
  raise 'state count' unless actual.size==expected.size
end
files.each { |p,h| raise 'identity drift' unless Digest::SHA256.file(p).hexdigest==h }
puts "#{expected.size*3} complete calculator states verified"
