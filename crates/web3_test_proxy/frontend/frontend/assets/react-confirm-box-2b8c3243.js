import{r as x}from"./react-dom-a94e3221.js";import{r as _}from"./react-077fff36.js";var p={};function y(i){if(i&&window){const a=document.createElement("style");return a.setAttribute("type","text/css"),a.innerHTML=i,document.head.appendChild(a),i}}function v(i){return i&&typeof i=="object"&&"default"in i?i.default:i}Object.defineProperty(p,"__esModule",{value:!0});var h=x,w=v(h),b=_,u=v(b);function g(i,a,l,e){return new(l=l||Promise)(function(n,o){function s(c){try{r(e.next(c))}catch(t){o(t)}}function d(c){try{r(e.throw(c))}catch(t){o(t)}}function r(c){c.done?n(c.value):new l(function(t){t(c.value)}).then(s,d)}r((e=e.apply(i,a||[])).next())})}function E(i,a){var l,e,n,o={label:0,sent:function(){if(1&n[0])throw n[1];return n[1]},trys:[],ops:[]},s={next:d(0),throw:d(1),return:d(2)};return typeof Symbol=="function"&&(s[Symbol.iterator]=function(){return this}),s;function d(r){return function(c){return function(t){if(l)throw new TypeError("Generator is already executing.");for(;o;)try{if(l=1,e&&(n=2&t[0]?e.return:t[0]?e.throw||((n=e.return)&&n.call(e),0):e.next)&&!(n=n.call(e,t[1])).done)return n;switch(e=0,(t=n?[2&t[0],n.value]:t)[0]){case 0:case 1:n=t;break;case 4:return o.label++,{value:t[1],done:!1};case 5:o.label++,e=t[1],t=[0];continue;case 7:t=o.ops.pop(),o.trys.pop();continue;default:if(!(n=0<(n=o.trys).length&&n[n.length-1])&&(t[0]===6||t[0]===2)){o=0;continue}if(t[0]===3&&(!n||t[1]>n[0]&&t[1]<n[3])){o.label=t[1];break}if(t[0]===6&&o.label<n[1]){o.label=n[1],n=t;break}if(n&&o.label<n[2]){o.label=n[2],o.ops.push(t);break}n[2]&&o.ops.pop(),o.trys.pop();continue}t=a.call(i,o)}catch(f){t=[6,f],e=0}finally{l=n=0}if(5&t[0])throw t[1];return{value:t[0]?t[1]:void 0,done:!0}}([r,c])}}}var k=function(){return"_"+Math.random().toString(36).substr(2,9)},m="confirm-box-root"+k(),C=function(a){var a=a.children,l=document.getElementById(m),e=document.createElement("div");return b.useEffect(function(){return l.appendChild(e),function(){return l.removeChild(e)}},[e,l]),h.createPortal(a,e)};y(`.confirm-box {
  z-index: 1000;
  position: absolute;
  left: 45%;
  top: 45%;
}
.confirm-box__content {
  z-index: 300;
  background-color: #fff;
  box-shadow: 0 4px 8px 0 rgba(0, 0, 0, 0.2);
  padding: 1em;
  border-radius: 5px;
  width: 300px;
  max-width: 400px;
}
.confirm-box__overlay {
  z-index: -1;
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background-color: rgba(0, 0, 0, 0.1);
}
.confirm-box__actions {
  display: flex;
  padding-top: 1em;
  justify-content: flex-end;
}
.confirm-box__actions > :not(:last-child) {
  margin-right: 1em;
}`);var N=function(t){function a(){f(!1),s(!0)}function l(){f(!1),s(!1)}var e,n,o,s=t.resolver,d=t.message,r=t.options,c=b.useState(!0),t=c[0],f=c[1];return t?u.createElement("div",{className:"confirm-box"},(e=(r||{}).classNames,n="confirm-box__content "+((e==null?void 0:e.container)||"")+`
    `,o=((e==null?void 0:e.buttons)||"")+" "+((e==null?void 0:e.confirmButton)||"")+`
    `,e=((e==null?void 0:e.buttons)||"")+" "+((e==null?void 0:e.cancelButton)||""),r!=null&&r.render?r.render(d,a,l):u.createElement("div",{className:n},u.createElement("span",null,d),u.createElement("div",{className:"confirm-box__actions"},u.createElement("button",{onClick:a,role:"confirmable-button",className:o},(o=r==null?void 0:r.labels)!==null&&o!==void 0&&o.confirmable?(o=r==null?void 0:r.labels)===null||o===void 0?void 0:o.confirmable:"Yes"),u.createElement("button",{onClick:l,role:"cancellable-button",className:e},(e=r==null?void 0:r.labels)!==null&&e!==void 0&&e.cancellable?(e=r==null?void 0:r.labels)===null||e===void 0?void 0:e.cancellable:"No")))),u.createElement("div",{className:"confirm-box__overlay",onClick:function(){r!=null&&r.closeOnOverlayClick&&(f(!1),s(!1))}})):null},B=function(i,a){return g(void 0,void 0,void 0,function(){var l;return E(this,function(e){switch(e.label){case 0:return[4,document.getElementById(m)];case 1:return e.sent()?[3,4]:[4,document.createElement("div")];case 2:return[4,(l=e.sent()).setAttribute("id",m)];case 3:e.sent(),document.body.appendChild(l),e.label=4;case 4:return[2,new Promise(function(n){n=u.createElement(N,{resolver:n,message:i,options:a}),n=u.createElement(C,null,n),w.render(n,document.getElementById(m))})]}})})},P=p.confirm=B;export{P as c};
