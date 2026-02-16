<template>
  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>

  <!-- Core Directives -->
  <div v-text="text"></div>
  <div v-html="html"></div>
  <div v-show="show"></div>
  <div v-if="condition">If</div>
  <div v-else-if="elseIfCondition">Else If</div>
  <div v-else>Else</div>
  <div v-for="(item, index) in items" :key="item.id">{{ index }}: {{ item.name }}</div>
  <div v-on="eventHandlers">Multiple Events</div>
  <div v-bind="objectAttrs">Multiple Attributes</div>
  <div v-once>{{ onceValue }}</div>
  <div v-pre>{{ not compiled }}</div>
  <div v-cloak>Hidden until compiled</div>
  <Component v-slot:default="slotProps">{{ slotProps.text }}</Component>
  
  <!-- v-bind variations -->
  <img :src="imageSrc" :alt="altText">
  
  <!-- v-on variations -->
  <button @click="handleClick">Click</button>
  <button @click.stop="handleClick">Stop</button>
  
  <!-- v-model variations -->
  <input v-model="textInput" type="text">
  
  <!-- Text Interpolation -->
  <div>{{ message }}</div>
  
  <!-- Edge Cases -->
  <div :[dynamicAttributeName]="attributeValue"></div>
  <div v-on:[dynamicEvent]="handler"></div>
</template>


