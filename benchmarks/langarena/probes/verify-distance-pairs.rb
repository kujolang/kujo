require 'json'
require 'digest'
raise 'usage: EVIDENCE_DIRECTORY' unless ARGV.size == 1
directory = ARGV[0]
manifest = JSON.parse(File.read(File.join(directory,'manifest.json')))
manifest.fetch('files').each { |path,hash| raise 'source/binary drift' unless Digest::SHA256.file(path).hexdigest == hash }
pairs = JSON.parse(File.read(File.join(directory,'pairs.json')))
reference = File.readlines(File.join(directory,'rust.out')).map { |line| JSON.parse(line) }
raise 'pair coverage' unless pairs.size == 400 && reference.size == 400
reference.each_with_index do |row,index|
  raise 'reference order' unless row[0] == index
  # A second formulation of the gram oracle: count every sliding byte window.
  grams = pairs[index].map do |string|
    counts = Hash.new(0)
    string.bytes.each_cons(4) { |window| counts[window] += 1 }
    counts
  end
  total = grams.sum(&:size)
  intersection = (grams[0].keys & grams[1].keys).sum { |key| [grams[0][key],grams[1][key]].min }
  score = total.zero? ? 0.0 : intersection.to_f / total
  raise 'gram oracle' unless (score-row[2]).abs <= 1e-12 && (score*1000).to_i == row[4]
end
valid_row = lambda do |row,index|
  expected = reference[index % 400]
  row.size == 6 && row[0,2] == [index / 400,index % 400] &&
    (row[2]-expected[1]).abs <= 1e-12 && (row[3]-expected[2]).abs <= 1e-12 &&
    row[4,2] == expected[3,2]
end
runs = JSON.parse(File.read(File.join(directory,'runs.json')))
raise 'runtime coverage' unless runs.map { |r| r['runtime'] }.sort == %w[interpreter jit vm]
runs.each do |run|
  raise 'runtime result' unless run['exit'] == 0 && run['signal'].nil?
  %w[stdout stderr].each do |stream|
    suffix = stream == 'stdout' ? 'out' : 'err'
    raise 'raw hash' unless Digest::SHA256.file(File.join(directory,run['runtime']+'.'+suffix)).hexdigest == run.fetch(stream+'_sha256')
  end
  rows = File.readlines(File.join(directory,run['runtime']+'.out')).map { |line| JSON.parse(line) }
  raise 'score coverage' unless rows.size == 800
  rows.each_with_index { |row,index| raise 'score mismatch' unless valid_row.call(row,index) }
  corrupted = rows.first.dup
  corrupted[2] += 0.0001 # Quantized checksum contribution remains zero.
  raise 'negative control accepted' if valid_row.call(corrupted,0)
  raise 'negative checksum changed' unless corrupted[4,2] == rows.first[4,2] && (corrupted[2]*1000).to_i == corrupted[4]
end
result = {status:'verified',score_observations:4800,rust_pairs:400,checksum_preserving_negative_controls:3,
          scope:'Kernel score equivalence only; production, generation/order, operation counts and Goose execution remain open'}
File.write(File.join(directory,'independent-verification.json'),JSON.pretty_generate(result))
puts JSON.generate(result)
