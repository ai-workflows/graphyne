- lambda functions
- interop with python
- exceptions
- string optimizations (global string map)

- early return from function calls
  - scrap the current implementation.
  - need a way to link the returned values from inside a called function's context to dependent ops in the caller context.
  - use a meta-graph. multiple meta-nodes can reference the same op node or val node.
  - a meta op node has a reference to a certain func op node, and the ids of the meta val nodes that are args and return values of the operation.
  - a meta val node has a list of references to the func val nodes it represents, as well as a list of ids to the meta op nodes that are dependent on it. It also has a reference to its actual value.
  - when an op is executed, the return values are used to create meta val nodes. If the val is an output value of its func, then the matching func val in the caller func is also saved, and can be retrieved recursively.
  - ex:
    ```graphite
    fn foo(a, b) -> c, d {
      c = a + b
      d = c * 2
    }
    
    fn main() {
      w, x = foo(1, 2)
      y = w * 2
      z = z * 3
    }
    ```
  - when foo is called, the op ```Add, [a, b], [c]``` is executed. Since c is an output of foo, the meta node for c references the func val nodes for c in foo and w in main.
  - meta op nodes for ```Mul, [c, 2], [d]``` and ```Mul, [w, 2], [y]``` are then both created and added as dependents of c's meta node and can be called in parallel.
  - whenever a value is calculated, it is checked to see if it is an output of the top-level function. If so, a callback is called to notify the caller that the value is ready.
  - each time a function is called, it is registered in a map with a guid for the call as the key and a reference to the func it represents as a value.
  - this map also has a queue for the output symbols of each function that are not known yet. When a value is calculated, if it is in the queue, it will be removed. If the queue is empty, the function is done and can be removed from the map.
  - the combination of the call id and func/op reference (ptr id) can be used to uniquely identify a meta node.

- serial execution
  - if a value node only has a single dependent, it is basically a temp value that does not need to be stored.
  - ex:
  ```graphite
  
    fn main() -> (z) {
      y = x + 2
      z = y * 3
    }
  ```
  - in this case, y is not needed outside of calculating z.
  - therefore, we never actually need to store the value of y. Instead, the value of y can just be used as a local rust variable to immediately calculate z, which is then stored.
  - Additionally, long chains of one-to-one dependencies can be executed directly without storing any intermediate values.
  - This can allow close to native performance for 'linear' calculations.
  - One aspect where this is particular useful is in the cases of lists and dicts.
  - Instead of having to clone the entire list or dict when it is mutated, if it is an intermediate value and is only used for this mutation, it can be mutated in place.
  - ex:
  ```graphite
    fn main() -> (list3) {
      list1 = [1, 2, 3]
      list2 = push(list1, 4)
      list3 = push(list2, 5)
    }
  ```
  - in this case, list1 and list2 are not used except to calculate their next value.
  - therefore, the 4 and 5 are pushed to list in place, and the values of list1 and list2 are never stored.
  - this can be implemented as a compiler optimization when a function is being stored in memory.
  - if a chain of value nodes with single dependents is found, the operations are converted into a single Serial operation.
  - the Serial op contains a vector of the opcodes with the value nodes of args and the index to inject the intermediate value into the args.
  - the Serial op is then executed as a single op with the temporary values being handled in place without being stored.
  - on the language side, this allows for writing the following "builder" style code, which is automatically optimizes into the the Serial op:
  ```graphite
    fn main() -> (my_list, my_num, my_dict) {
  
      my_list = [1, 2, 3]
        => push(4)
        => push(5);
  
      my_num = 1 
        => add(2) 
        |i| => get(my_list, i);
  
      my_dict = {}
        => set("a", 1)
        => set("b", 2)
        => set("c", 3);
  
      doubled_list = x in my_list -> x * 2
  
      doubled_vals = k, v in my_dict -> (res) {
        res = v * 2
      }
  
      sum = 0 |last| |=> (x in my_list) -> x + last
    }
  ```
