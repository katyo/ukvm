<TopAppBar bind:this={topAppBar} varian="fixed">
  <Row>
    <Section>
      <Title>UBC UI</Title>
    </Section>
    <Section align="center" toolbar>
      <SegmentedButton segments={pages} let:segment={page} singleSelect bind:selected={active_page} key={(page) => page.name}>
        <Segment segment={page} title={page.name} class={page === active_page ? "mdc-top-app-bar__action-item" : ""}>
          <Icon class="material-icons">{page.icon}</Icon>
          <Label>{page.title}</Label>
        </Segment>
      </SegmentedButton>
    </Section>
    <Section align="end" toolbar>
      {#each leds as id}
      <Button disabled>
        <Icon class="material-icons">radio_button_{leds_status[id] ? "checked" : "unchecked"}</Icon>
        {id} LED
      </Button>
      {:else}
      <Button disabled>No LEDs</Button>
      {/each}
      {#each buttons as id}
        <Button on:click={() => { button_press_id = id; button_press_confirm = true; }}>
          {id}
        </Button>
      {:else}
        <Button disabled>No Buttons</Button>
      {/each}
    </Section>
  </Row>
</TopAppBar>

<AutoAdjust {topAppBar}>
  <h5>Always Collapsed</h5>
</AutoAdjust>

<Dialog bind:open={button_press_confirm} aria-labelledby="simple-title" aria-describedby="simple-content">
  <Title id="simple-title">Confirm action</Title>
  <Content id="simple-content">You pressed {button_press_id} button. Are you sure?</Content>
  <Actions>
    <Button>
      <Label>No</Label>
    </Button>
    <Button on:click={() => { button_press(button_press_id); }}>
      <Label>Yes</Label>
    </Button>
  </Actions>
</Dialog>

<script lang="ts">
 //import { LedId, ButtonId, capabilities, button_press, on_led_status } from './api/fake';
 import { LedId, ButtonId, capabilities, button_press, on_led_status } from './api/http';

 import type { TopAppBarComponentDev } from '@smui/top-app-bar';
 import TopAppBar, { Row, Section, Title, AutoAdjust } from '@smui/top-app-bar';
 //import Tab from '@smui/tab';
 //import TabBar from '@smui/tab-bar';
 import Button from '@smui/button';
 import IconButton from '@smui/icon-button';
 import SegmentedButton, { Segment } from '@smui/segmented-button';
 import { Icon, Label } from '@smui/common';
 import Dialog, { Title, Content, Actions } from '@smui/dialog';

 //import { onMount } from 'svelte';

 let topAppBar: TopAppBarComponentDev;

 export interface LedsStatus {
   [id: LedId]: boolean,
 }

 let leds: LedId[] = [];
 let leds_status: LedsStatus = {};

 let buttons: ButtonId[] = [];
 let button_press_confirm: boolean = false;
 let button_press_id: ButtonId?;

 const enum PageId {
   TextConsole = "text-console",
   HostTextConsole = "host-text-console",
   HostGraphicsConsole = "host-graphic-console",
 }

 interface Page {
   name: string,
   title: string,
   icon: string,
 }

 let pages: Page[] = [
   { name: PageId.TextConsole, icon: 'terminal', title: 'Text Console' },
   { name: PageId.HostTextConsole, icon: 'terminal', title: 'Host Test Console' },
   { name: PageId.HostGraphicsConsole, icon: 'personal video', title: 'Host Video Console' },
 ];

 let active_page = pages[0];

 capabilities().then(capabilities => {
   leds = capabilities.leds;
   buttons = capabilities.buttons;
 });

 on_led_status((id, status) => {
   leds_status[id] = status;
 });
</script>
