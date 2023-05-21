rsync \
	--verbose \
	--archive \
	--update \
	--delete \
	--exclude 'target' \
	--exclude '.git' \
	--exclude '.gitignore' \
	--exclude '.DS_Store' \
	--exclude 'sync.sh' \
	. dendrite@n0g.rip:yt-rss