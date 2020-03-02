# CHARON - The Boards-backupper

**Hello!**

If you're here, you must know that things aren't looking good. The Boards is
shutting down in a few days and Riot is planning to wipe the archives. To at
least preserve *some* of this great (and terrible) site, I wrote this program
which can be used to basically save everything from the forum.

## How does it work?

When you start up CHARON, it will ask you a few simple questions to know which
version of the Boards you wish to save. Then it will create a folder where the
executable is named "backup_{REGION}_{LANGUAGE}". After that, it will fetch the
posts and comments of whoever you named and saves them in this folder.
Meanwhile, it also collects every single name it encounters. Then it takes one
of those names and repeats this whole process. As you can probably imagine,
this will eventually exhaust every single topic and comment.

## How quick is it?

Both very quick and quite slow at the same time. The program itself uses
optimized libraries and Rust's already excellent speed. However, since you're
downloading from the internet, bandwidth and the inherent slowness of requests
makes the program run much slower than it could.

To give you a few concrete numbers:

The Hungarian Boards contains 21,000 topics from about 13,000 users.
Downloading this took about 2 and a quarter hours on my Ryzen 5 3600 processor
and 600 Mbps internet connection.

## How will I be able to access this data?

I already have a working version of the Hungarian dataset, which can be reached
[here](https://lolarchivum.github.io). If you click on the second "OK" button,
you can see that over 20 thousand posts load almost immediately.

The project can be reached
[here](https://github.com/lolarchivum/lolarchivum.github.io/), it shouldn't
take much to modify it to be able to use any dataset.

However, either way, once we have the content saved up, we can take as much
time as wish to release it again, the first priority should be to back it up.

## How do I build the project?

Either you don't and just download the newest release from the Releases tab
above or you need to install [Rust](https://www.rust-lang.org/), navigate into
the folder where you downloaded the code and issue

`cargo run --release`

which will download the dependencies and run the program.

## How can you help?

While the program can handle the EUNE/HU region well, I haven't tested it
anywhere else, so if you can just do that, that's already very helpful.

I sadly don't own a Windows PC, so I wasn't able to turn the application into
an exe. So if you happen to have Rust installed on a Win PC and would be
willing to package it up and make a pull request, I'd happily add it to the
Releases page.

Also, in the unlikely case you need to restart the application, it sadly has no
idea how much has it already processed, so it'll start again from scratch. This
is obviously undesirable, but I wanted to quickly whip up a working version. If
you can manage to patch this in, please open a pull request.

And, to be frank, any kind of improvement is very much appreciated. **The Boards
might die, but its content doesn't have to. It all comes down to us.**
