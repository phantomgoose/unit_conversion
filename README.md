# What's this?

A solution for a [Jane Street](https://www.janestreet.com) mock interview question, in Rust.

TLDR version is that our goal is to create a procedure that allows unit conversions given an initial set of facts not known at compile time[^1].

There's no guarantee that a solution exists, and converting from one unit to another may require multiple "hops" between various units.

```text
example facts:
m = 3.28 ft
ft = 12 in
hr = 60 min
min = 60 sec

example queries:
2 m = ? in --> answer = 78.72
13 in = ? m --> answer = 0.330 (roughly)
13 in = ? hr --> "not convertible!"
```

See the following video for more context: https://youtu.be/V8DGdPkBBxg. This problem is also similar to the [Evaluate Division](https://leetcode.com/problems/evaluate-division) LC question.

# Solution

For our solution, we're going to implement a cyclic graph to capture the relationships between known Units with conversion rates stored as part of the edge metadata.

May or may not be _slightly_ overengineered, as this is part of my "learning Rust" series of projects. Feedback and suggestions are super welcome!

___

[^1]: Note that for the purposes of demoing the functionality, we _do_ create a static set of conversion facts on startup.
