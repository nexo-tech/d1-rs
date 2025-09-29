# Why We Chose Rust for Database Abstraction in 2025 (And Why It Was the Right Bet)

*The contrarian technical decision that enabled everything else*

## The Obvious Choice Was TypeScript

In September 2025, if you wanted to build something for Cloudflare Workers, the obvious choice was TypeScript. The entire ecosystem was built around JavaScript. The documentation assumed JavaScript. The community examples were in JavaScript. Most importantly, the path of least resistance was JavaScript.

But we chose Rust instead. And three months later, when we had built something that couldn't exist in any other language, it became clear why that contrarian choice was the foundation of everything that followed.

## The 2025 Serverless Landscape: JavaScript Everywhere

The serverless ecosystem in 2025 was dominated by a simple assumption: edge computing meant JavaScript. Cloudflare Workers, Vercel Edge Functions, and Deno Deploy all optimized for V8 execution. The mental model was straightforward: your edge code should be fast-starting JavaScript that handles HTTP requests and calls APIs.

For most use cases, this made perfect sense. You didn't need complex data processing at the edge. You didn't need sophisticated type systems. You just needed fast HTTP handlers that could forward requests and aggregate responses.

But we were trying to build something different: a stateful application with complex database operations, running at the edge, with guarantees about correctness that JavaScript couldn't provide.

## The Database Problem at the Edge

Traditional serverless wisdom said: keep your edge functions stateless. Put your database somewhere else. Make API calls back to your primary region when you need to read or write data. Accept the latency penalty for the simplicity.

Cloudflare D1 changed that calculus. Suddenly, you could have an actual database at the edge. But there was a catch: it wasn't quite SQLite, and it wasn't quite a traditional database API. It was something in between - a JavaScript API that spoke to SQLite, with WebAssembly under the hood, and constraints that no existing ORM understood.

The existing Rust database libraries were designed for long-lived server processes with connection pools. The existing JavaScript ORMs assumed they could introspect schema at runtime. Neither model fit the edge computing paradigm where your code needs to start instantly and run efficiently in a heavily constrained environment.

## The Type Safety Imperative

Here's the thing about database code: when it fails, it fails catastrophically. A typo in a column name doesn't just return null - it brings down your entire service. A type mismatch doesn't give you a helpful error message - it corrupts your data.

In JavaScript, these failures happen at runtime, in production, with real user data. The feedback loop is: deploy, fail, debug, fix, redeploy. For a calendar booking system handling real appointments and money, that feedback loop was unacceptable.

Rust offered something different: the possibility of catching these errors at compile time. Not just the obvious errors, but the subtle ones. Not just today's errors, but the errors that would emerge months later when someone refactored the user model and forgot to update the booking queries.

The question was: could we build a database abstraction layer that actually delivered on that promise?

## The WASM Bet

WebAssembly in 2025 was promising but immature. Most languages had basic WASM support, but the ecosystem was fragmented. Tooling was inconsistent. Performance characteristics were unpredictable. For most teams, betting on WASM was betting on the future instead of shipping today.

But Rust's WASM story was different. The toolchain was mature. The performance was predictable. Most importantly, Rust's ownership model mapped naturally to WASM's constraint-based execution environment.

This meant we could write our database code once, in Rust, and compile it for two completely different targets: native binaries for development (using SQLite directly) and WASM for production (using Cloudflare's D1 API). Same code, same guarantees, different execution environments.

No other language could offer this dual-target capability with the same performance and safety characteristics.

## The Performance Reality

Edge computing is fundamentally about performance. Your code needs to start in milliseconds, not seconds. Memory usage is capped at 128MB. CPU time is limited and expensive. There's no room for garbage collection pauses or dynamic dispatch overhead.

JavaScript is fast for many workloads, but it's unpredictably fast. JIT compilation means your first few requests are slow while V8 optimizes your code. Garbage collection means periodic pauses you can't control. Dynamic typing means runtime checks on every operation.

Rust offered predictable performance. Zero-cost abstractions meant our high-level database code could compile down to machine code that was as efficient as hand-written C. No garbage collector meant no unpredictable pauses. Static typing meant no runtime type checks.

For a database abstraction layer, this performance predictability was crucial. Database operations are often the bottleneck in web applications. Making them unpredictably slow would defeat the entire purpose of edge computing.

## The Innovation Opportunity

But the real reason we chose Rust wasn't just about avoiding the problems with JavaScript. It was about the positive capabilities that only Rust could provide.

Rust's trait system enabled compile-time polymorphism in ways that other languages couldn't match. We could define database operations as traits, implement them differently for different backends, and have the compiler generate optimized code for each case. No runtime dispatch. No performance penalty. Just type-safe abstractions that disappeared at compile time.

Rust's procedural macros enabled code generation that was both powerful and safe. We could analyze Rust structs at compile time, generate database schema migrations, and create type-safe query builders - all without runtime reflection or string manipulation.

Rust's ownership system enabled memory safety without garbage collection, which was perfect for the constrained edge environment. We could process large datasets, build complex in-memory representations, and know exactly when memory would be freed.

No other language could offer this combination of capabilities.

## The Contrarian Choice Pays Off

Three months after that initial commit, when we had built a database ORM that could generate type-safe queries at compile time, work seamlessly across native and WASM targets, and provide zero-overhead abstractions for complex database operations, the early architectural decision became clear.

We hadn't just chosen Rust because it was better than JavaScript for this specific use case. We had chosen Rust because it enabled us to build something that was literally impossible in any other language.

The compile-time query safety, the dual-target architecture, the zero-overhead abstractions - none of these features could have been built in TypeScript, Go, Python, or even other systems languages like C++. They required Rust's specific combination of performance, safety, and expressiveness.

## The Hidden Cost of Obvious Choices

The TypeScript path would have been easier in the short term. We could have shipped a working calendar application in a few weeks instead of a few months. We could have used existing libraries, followed established patterns, and avoided the complexity of cross-compilation and WASM tooling.

But we would have built something fundamentally different. A JavaScript ORM with runtime safety checking instead of compile-time guarantees. A single-target system instead of a truly cross-platform abstraction. A conventional database layer instead of a revolutionary approach to type safety.

The obvious choice would have given us a working product faster. The contrarian choice gave us a foundational technology that opened up possibilities we didn't even know existed.

## The Broader Lesson

In 2025, choosing Rust for edge computing was contrarian. Most teams chose JavaScript because that's what the platforms optimized for. Most tutorials assumed JavaScript because that's what most developers knew. Most investors preferred JavaScript because it meant faster time-to-market.

But contrarian technical choices are often contrarian for a reason: they require you to solve harder problems in exchange for capabilities that don't exist in the mainstream approach.

The question isn't whether Rust is "better" than JavaScript for serverless applications in general. The question is whether the specific capabilities that Rust enables - compile-time safety, predictable performance, cross-target compilation - are worth the additional complexity for your specific use case.

For us, building a database abstraction layer that needed to work reliably with user data across multiple execution environments, the answer was unambiguously yes.

The git history shows the evolution from that initial contrarian bet to a mature system that vindicated the choice. Sometimes the best technical decisions are the ones that seem obviously wrong to everyone else.

---

*The architectural decisions documented in commit 8377d43 established patterns that enabled every subsequent innovation. The complete technical evolution is tracked across 185 commits in the project repository.*