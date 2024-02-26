# Async/Await

`Async/Await` lets the programmer write code that looks like normal synchronous code. But the compiler then turns the code into asynchronous code. The `async` keyword can be used in a function signature to turn a synchronous function into an asynchronous function that returns a future:

The way `async/await` works is that programmers write code that looks like synchronous code. But the compiler then turns the code into asynchronous code. `Async/Await` is based on two keywords: `async` and `await`. During compilation, any code block wrapped inside the `async` keyword is converted into a state machine in the form of a `Future`.

As a simple example, the following async function `one_fn` may be compiled into `compiled_one_fn`, which is a function that returns a `Future`.

```rust
async fn one_fn() -> u32 {
    1
}

fn compiled_one_fn() -> impl Future<Output = u32> {
    future::ready(1)
}
```

To gain a better intuition for how asynchronous code gets converted into a state machine, let’s look at a more complex `async` function. We are going to convert the `notify_user` method below into a state machine that implements the `Future` trait.

```rust
async fn notify_user(user_id: u32) {
	let user = async_fetch_user(user_id).await;
	if user.group == 1 {
		async_send_email(&user).await;
	}
}
```

The method above first fetches the user’s information. It then sends an email if the user’s group matches `1`.

If we think about the function as a state machine, here are its possible states:

- **Unpolled**: the start state of the function
- **FetchingUser**: the state when the function is waiting for `async_fetch_user(user_id)` to complete
- **SendingEmail**: the state when the function is waiting for `async_send_email(user)` to complete
- **Ready**: the end state of the function.

Each point represents a pausing point in the function. The state machine we are going to create implements the `Future` trait. Each call to the future’s `poll` method performs a possible state transition.

The compiler creates the following enum to track the state of the state machine (note that my examples are for demonstration purposes and not what the compiler actually generates)

```rust
enum State {
	Unpolled,
	FetchingUser,
	SendingEmail,
	Ready
}
```

Next, the compiler generates the following struct to hold all the variables the state machine needs.

```rust
struct NotifyUser {
	state: State,
	user_id: u32,
	fetch_user_fut: Option<impl Future<Output = User>>,
	send_email_fut: Option<impl Future<Output = ()>>,
	user: Option<User>
}
```

To track the progress of `async_fetch_user(user_id).await` and `async_send_email(&user).await`, the state machine stores the `async_fetch_user`'s state machine inside the `fetch_user_fut` field and stores the `async_send_email`'s state machine inside the `send_email_fut` field.

Note that `fetch_user_fut` and `send_email_fut` are both `Option`s. This is because the state machine won’t be initiated until the `NotifyUser` state machine reaches there. In the case of `send_email_fut`, the state machine may never be initiated in the case that `[user.group](<http://user.group>)` is not `1`.

Conceptually, `fetch_user_fut` and `send_email_fut` are like children state machines that make up a bigger state machine that is the `NotifyUser`.

Now that we have a state machine, let’s implement the `Future` trait:

```rust
impl Future for NotifyUser {
	type Output = ();

	fn poll(&mut self, cx: &mut Context) -> Poll<()> {
		loop {
			match self.state {
				State::Unpolled => { todo!() },
				State::FetchingUser => { todo!() },
				State::SendingEmail => { todo!() },
				State::Ready => { todo!() };
			}
		}
	}
}
```

The `poll` method starts a `loop` because in the case that one of the states isn’t blocked, the state machine can perform multiple state transitions in a single `poll` call. This reduces the number of `poll` calls the executor needs to make.

Now, let’s look at how each state performs the state transition.

When we initialize `NotifyUser`, its `state` is `State::Unpolled`, which represents the starting state. When we `poll` `NotifyUser` for the first time, it calls `async_fetch_user` to instantiate and store the `fetch_user_fut` state machine.

It then transitions its `state` to `State::FetchingUser`. Note that this code block doesn’t return `Poll::Pending`. This is because none of the executed code is blocking, so we can go ahead and execute the handle for the next state transition.

```rust
State::Unpolled => {
	self.fetch_user_fut = Some(async_fetch_user(self.user_id));
	self.state = State::FetchingUser;
}
```

When we get to the `FetchinUser` state, it `poll`s the `fetch_user_fut` to see if it’s ready. If it’s `Pending`, we return `Poll::Pending`. Otherwise, `NotifyUser` can perform its next state transition. If `self.user.group == 1`, it needs to create and store the `fetch_user_fut` state machine and transition the state to `State::SendingEmail`. Otherwise, it can transition its state to `State::Ready`.

```rust
State::FetchingUser => {
	match self.fetch_user_fut.unwrap().poll(cx) {
		Poll::Pending => return Poll::Pending,
		Poll::Ready(user) => {
			self.user = Some(user);
			if self.user.group == 1 {
				self.fetch_user_fut = Some(async_send_email(&self.user));
				self.state = State::SendingEmail;
			} else {
				self.state = State::Ready;
			}
		}
	}
}
```

If the state is `SendingEmail`, it polls `send_email_fut` to check if it’s ready. If it is, it transitions the state to `State::Ready`. Otherwise, it returns `Poll::Pending`.

```rust
State::SendingEmail => {
	match self.send_email_fut.unwrap().poll(cx) {
		Poll::Pending => return Poll::Pending,
		Poll::Ready(()) => {
			self.state = State::Ready;
		}
	}
}
```

Finally, if the state is `Ready`, `NotifyUser` returns `Poll::Ready(())` to indicate that the state machine is complete.

```rust
State::Ready => return Poll::Ready(());
```

Here is the full code:

```rust
enum State {
	Unpolled,
	FetchingUser,
	SendingEmail,
	Ready
}

struct NotifyUser {
	state: State,
	user_id: u32,
	fetch_user_fut: Option<impl Future<Output = User>>,
	send_email_fut: Option<impl Future<Output = ()>>,
	user: Option<User>
}

impl Future for NotifyUser {
	type Output = ();

	fn poll(&mut self, cx: &mut Context) -> Poll<()> {
		loop {
			match self.state {
				State::Unpolled => {
						self.fetch_user_fut = Some(async_fetch_user(self.user_id));
						self.state = State::FetchingUser;
				},
				State::FetchingUser => {
						match self.fetch_user_fut.unwrap().poll(cx) {
							Poll::Pending => return Poll::Pending,
							Poll::Ready(user) => {
								self.user = Some(user);
								if self.user.group == 1 {
									self.fetch_user_fut = Some(async_send_email(&self.user));
									self.state = State::SendingEmail;
								} else {
									self.state = State::Ready;
								}
							}
						}
				},
				State::SendingEmail => {
					match self.send_email_fut.unwrap().poll(cx) {
						Poll::Pending => return Poll::Pending,
						Poll::Ready(()) => {
							self.state = State::Ready;
						}
					}
				},
				State::Ready => return Poll::Ready(());
			}
		}
	}
}
```
