<script lang="ts">
	import ProfileHeader from '$lib/components/shared/ProfileHeader.svelte';

	let { data, children } = $props();
</script>

<ProfileHeader profile={data.profile} did={data.did} showFollow={true} />
{@render children()}
