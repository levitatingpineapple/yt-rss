# Simple Youtube RSS Server

While YT technically still has a legacy RSS feed service, it does not actually contain any videos.
This server provides an RSS feed for a given channel. It relies on [yt-dlp](https://github.com/yt-dlp/yt-dlp) for retrieving the actual video instead of just embedding YT player.

## Install Locally ([Apple Silicon](https://support.apple.com/en-us/HT211814))

There is a published apple arm binary on [Homebrew]( https://brew.sh) for running it locally.

```bash
# Make sure the package manager is up to date
brew update

# Install the server
brew tap levitatingpineapple/formulae
brew install yt-rss

# Start the service
brew tap homebrew/services
brew services start yt-rss

# Check that the service has been started
brew services list
```

## Usage

By default the server runs on port `8080`.\
You can subscribe using the handle of the channel in rss:
```
http://localhost:8080/@Fireship
```

>:warning: Requires reader which supports [rss enclosures](https://en.wikipedia.org/wiki/RSS_enclosure)\
>Like the upcoming [Feed Radar](https://github.com/levitatingpineapple/feed-radar/) for example:

![Feed Radar](https://github.com/levitatingpineapple/feed-radar/raw/main/.readme/app.webp)