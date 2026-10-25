<script lang="ts">
    import { lastFocus } from "../stores";
    export let timestamp: string;

    let datetime = new Date(timestamp);

    /**
     * How long ago a moment was, in words. Chinese reads "10 个月前" rather
     * than "10 months ago", and the near-term cases have their own words
     * instead of a number.
     */
    function relativeDate(now: number): string {
        let seconds = Math.round((datetime.getTime() - now) / 1000);
        let ago = seconds < 0;
        let value = Math.abs(seconds);

        if (value < 45) {
            return "刚刚";
        }
        if (value < 90) {
            return "1 分钟前";
        }
        if (value < 3600) {
            return `${Math.round(value / 60)} 分钟前`;
        }
        if (value < 7200) {
            return "1 小时前";
        }
        if (value < 86400) {
            return `${Math.round(value / 3600)} 小时前`;
        }
        if (value < 172800) {
            return "昨天";
        }
        if (value < 2592000) {
            return `${Math.round(value / 86400)} 天前`;
        }
        if (value < 5184000) {
            return "1 个月前";
        }
        if (value < 31536000) {
            return `${Math.round(value / 2592000)} 个月前`;
        }
        if (value < 63072000) {
            return "1 年前";
        }
        return `${Math.round(value / 31536000)} 年前`;
    }
</script>

<span class="timestamp" title={datetime.toLocaleString()}>
    {relativeDate($lastFocus)}
</span>

<style>
    .timestamp {
        color: var(--ctp-subtext1);
        white-space: nowrap;
        pointer-events: auto;
    }
</style>
