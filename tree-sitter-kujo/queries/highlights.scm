; Kujo highlighting queries

"func" @keyword
"let" @keyword
"const" @keyword
"mut" @keyword
"if" @keyword
"else" @keyword
"for" @keyword
"in" @keyword
"while" @keyword
"return" @keyword
"async" @keyword
"await" @keyword
"struct" @keyword
"match" @keyword
"case" @keyword

(function_definition
  name: (identifier) @function)

(call_expression
  function: (identifier) @function.call)

(variable_declaration
  name: (identifier) @variable)

(parameter_list
  (identifier) @parameter)

(identifier) @variable
(number) @number
(string) @string
(comment) @comment
