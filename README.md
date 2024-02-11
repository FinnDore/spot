# Spot

Spot is an api wrapper for the [Spotify Web API](https://developer.spotify.com/web-api/), allowing you to get your currently playing song and your shorterm top songs. Requests are serveed from a in memory cache that has a TTL of 10 seconds.

Spot also allows the pause, play and skip to the previous or next song for the connected account, via an API token.

## Routes

| path                    | description                                                | Example Payload / Response                             |
| ----------------------- | ---------------------------------------------------------- | ------------------------------------------------------ |
| `/top-songs`            | Lists the top songs                                        | [Example](./reference/spot/top-songs.json)             |
| `/`                     | Returns the currently playing song                         | [Example](./reference/spot/current-song.json)          |
| `/player/:player_state` | Changes the current player state for the connected account | `player_state`: `play`, `pause`, `next` and `previous` |
