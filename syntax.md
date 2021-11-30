### Basics

Very rust-like.

Lines optionally end with semicolons, unless there's more than one statement, in which case semicolons are mandatory to separate them.

#### Defining variables
```
let x           // declaration without assignment
                // (legal unless you try to use it before initializing)
let x = 3       // plain assignment (type inferred as <numeric>)
let x = 3.14    // plain assignment (type inferred as <float>)
let x = 3u32    // plain assignment (type specified as u32)
let x: u32 = 3  // plain assignment (type specified as u32)
```
#### Strings
```
// strings can be surrounded with either type of quote
let str1 = 'single quotes'
let str2 = "double quotes"
// all strings are unicode
"A ʯ Ϭ א ஆ ᛟ せ 漢 겅 ✦ ✨"
```
#### Flow Control
```
if a == 0 { thing1() }
else if b == 0 { thing2() }
else { thing3() }

loop {
    im_stuck_in_another_gosh_dang_vortex();
    // oooghh that medicine man tricked me
}

for x in 0..10 { i_can_count(x) }

// all blocks return a value
let x = if a == 0 { y } else { z }
let a = {
    do_some_thing_here();
    let b = c + d;
    d / e
}
```
#### Lists
```
// lists must be homogenous
let arr = [1, 2, 3]
let arr2 = [1, "a", []]
^ ERROR
```
#### List Comprehensions (?)
```
[ x*x for x in [ y.z() for y in q() ] ]
```
#### Tuples
```
(1, "a") is a (<number>, string)
(note that number is undifferentiated unless specified
let x: (String, [u32])
x is a tuple of type (String, [u32])
(the list size isnt specified)
```

### Functions

#### Calling functions in function namespace
```
mkdir(params)
```

#### Paths
```
p"/some/path"
```

#### Calling a path as an executable
```
p"/some/path"(params)
```

#### Loading path as an executable and adding it to the namespace:
```
p"/some/path".load()
```
