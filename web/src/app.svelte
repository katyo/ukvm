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
        <Icon class="material-icons">radio_button_{leds_state[id] ? "checked" : "unchecked"}</Icon>
        {id} LED
      </Button>
      {:else}
      <Button disabled>No LEDs</Button>
      {/each}
      {#each buttons as id}
        <Button on:press={() => { set_button_state(id, true); }} on:release={() => { set_button_state(id, false); }}>
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

<script lang="ts">
 //import { LedId, ButtonId, capabilities, button_state, set_button_state, on_button_state, led_state, on_led_state } from './api/fake';
 import { LedId, ButtonId, capabilities, button_state, set_button_state, on_button_state, led_state, on_led_state } from './api/http';

 import type { TopAppBarComponentDev } from '@smui/top-app-bar';
 import TopAppBar, { Row, Section, Title, AutoAdjust } from '@smui/top-app-bar';
 //import Tab from '@smui/tab';
 //import TabBar from '@smui/tab-bar';
 import Button from '@smui/button';
 import IconButton from '@smui/icon-button';
 import SegmentedButton, { Segment } from '@smui/segmented-button';
 import { Icon, Label } from '@smui/common';
 import Dialog, { Title, Content, Actions } from '@smui/dialog';

 import { onMount } from 'svelte';

 let topAppBar: TopAppBarComponentDev;

 export interface LedsState {
     [id: LedId]: boolean,
 }

 export interface ButtonState {
     [id: ButtonId]: boolean,
 }

 let leds: LedId[] = [];
 let leds_state: LedsState = {};

 let buttons: ButtonId[] = [];
 let buttons_state: ButtonsState = {};

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

 onMount(() => {
     capabilities().then(capabilities => {
         leds = capabilities.leds;
         buttons = capabilities.buttons;

         for (const id of leds) {
             led_state(id).then(state => {
                 leds_state[id] = state;
             });
         }

         for (const id of buttons) {
             button_state(id).then(state => {
                 buttons_state[id] = state;
             });
         }

         on_led_state((id, state) => {
             leds_state[id] = state;
         });

         on_button_state((id, state) => {
             buttons_state[id] = state;
         });
     });
 });
</script>
