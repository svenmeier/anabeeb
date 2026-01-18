(function() {

    window.anabeeb = {
        position: (node, left, top) => {
            node.style.left = `${left}px`;
            node.style.top = `${top}px`;
        },

        post: (path) => {
            return request(path, {
                method: 'POST',
            });
        },

        get: (path) => {
            return request(path);
        },

        remove: (id) => {
            disposition.elements[id] = undefined;

            document.getElementById(id)?.remove();
        },

        change: (id, values, optionalValues) => {
            disposition.elements[id] = {
                ...(optionalValues || {}),
                ...(disposition.elements[id] || {}),
                ...values,
            }
            initNode(id, disposition.elements[id]);
        }
    };

    async function request(path, options = {}) {
        const res = await fetch(`http://${disposition.uri}${path}`, options)
            .catch(showError);
        if (!res?.ok) {
            throw new Error(`request failed ${res?.status} ${res?.statusText}`);
        }
        return res.json();
    }

    function activate(id, active) {
        const action = active ? 'activate' : 'deactivate';

        anabeeb.post(`/element/${id}/${action}`);
    }

    function change(id, value) {
        anabeeb.post(`/element/${id}/change/${value}`);
    }

    function updateElement(id, node, element) {
        const listening = node.dataset?.ui_listening;

        const input = node.querySelector('input');
        switch (element.type) {
            case 'Coupler':
            case 'MidiAction':
            case 'Captor':
                input.checked = (element.active === true);
                if (!listening) node.addEventListener("click", () => activate(id, input.checked));
                break;
            case 'Combination':
                input.checked = (element.active === true);
                if (!listening) node.addEventListener("click", () => {
                    input.checked = true;
                    activate(id, true);
                });
                break;
            case 'MidiRange':
            case 'Memory':
                input.min = element.min;
                input.max = element.max;
                input.value = element.value;
                if (!listening) node.addEventListener("input", () => change(id, Number(input.value)));
                break;
        }

        node.dataset.ui_listening = "true";
    }

    function showError(error) {
        console.error(error);

        let errors = document.getElementById('errors');
        errors.showModal();
    }

    function initNode(id, element) {
        let node = document.getElementById(id);
        if (!node || node.dataset.element_template !== element.template) {
            document.getElementById(id)?.remove();

            const parentNode = document.querySelector(".elements");

            const template = document.querySelector(`[data-template="${element.template}"]`);
            if (!template) {
                console.warn(`No template "${element.template}"`);
                return;
            }

            node = template.content.firstElementChild.cloneNode(true);
            node.id = id;
            node.dataset.element_template = element.template;
            parentNode.appendChild(node);

            anabeeb.get(`/element/${id}`)
                .then(json => {
                    updateElement(id, node, json.elements[id]);
                });
        }

        anabeeb.position(node, element.x, element.y);
        node.style.scale = element.scale || 1;

        const input = node.querySelector('input');
        const label = node.querySelector('label');
        if (input && label) {
            input.id = id + '-input';
            label.setAttribute('for', input.id);
        }
        label.textContent = element.label;
    }

    function initWebsocket() {
        const socket = new WebSocket(`ws://${disposition.uri}/ws`, "anabeeb");
        socket.onmessage = (event) => {
            const data = JSON.parse(event.data);

            if (data.elements) {
                Object.keys(data.elements).forEach(id => {
                    const node = document.getElementById(id);
                    if (node) {
                        updateElement(id, node, data.elements[id]);
                    }
                });
            }
        };
    }

    window.addEventListener('load', () => {
        Object.entries(disposition.elements).forEach(([id, element]) => initNode(id, element));

        initWebsocket();
    });
})();