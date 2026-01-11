(function() {

    let editor;

    let editing = undefined;
    let binding = false;
    let moving = undefined;

    let elements = undefined;

    function grid(n) {
        return Math.round(n / disposition.grid) * disposition.grid;
    }

    function bindStart(node) {
        const id = node.id;
        anabeeb.post(`/binding/${id}/start`);
    }

    function bindCancel(node) {
        const id = node.id;
        anabeeb.post(`/binding/${id}/cancel`);
    }

    function bindConfirm(node) {
        const id = node.id;
        anabeeb.post(`/binding/${id}/confirm`);
    }

    function getTemplates(elementType) {
        let type = undefined;
        switch(elementType) {
            case 'Coupler':
            case 'MidiAction':
            case 'Captor':
            case 'Combination':
                type = 'checkbox';
                break;
            case 'MidiRange':
                type = 'range';
                break;
            case 'Memory':
                type = 'number';
                break;
        }

        return [...document.querySelectorAll("[data-template]")]
            .filter(template => {
                return type ? template.content.querySelector(`input[type="${type}"]`)
                     : !template.content.querySelector(`input`);
            })
            .map(template => template.dataset.template);
    }

    function initEditor() {
        const root = document.getElementById('editor');
        editor = {
            root,
            id: root.querySelector('[data-editor-id]'),
            label: root.querySelector('[data-editor-label]'),
            template: root.querySelector('[data-editor-template]'),
            elements: root.querySelector('[data-editor-elements]'),
            uri: root.querySelector('[data-editor-uri]'),
            grid: root.querySelector('[data-editor-grid]'),
            scale: root.querySelector('[data-editor-scale]'),
            move: root.querySelector('[data-editor-move]'),
            bind: root.querySelector('[data-editor-bind]'),
            remove: root.querySelector('[data-editor-remove]'),
            download: root.querySelector('[data-editor-download]'),
            add: root.querySelector('[data-editor-add]'),
        };

        editor.root.addEventListener("mousedown", (event) => {
            event.stopImmediatePropagation();
        });

        editor.label.addEventListener('input', () => {
            const value = editor.label.value.trim();
            editing.querySelector('label').textContent = value;
            disposition.elements[editing.id].label = value;
        });

        editor.template.addEventListener('change', () => {
            const value = editor.template.value;
            const element = disposition.elements[editing.id];
            element.template = value;
            anabeeb.added(editing.id, element);
            initElement(editing.id, element);
        })
        editor.scale.addEventListener('change', () => {
            const value = editor.scale.value;
            editing.style.scale = value;
            const element = disposition.elements[editing.id];
            element.scale = value;
        })

        editor.uri.addEventListener('change', () => {
            const value = editor.uri.value;
            disposition.uri = value;
        })
        editor.grid.addEventListener('change', () => {
            const value = editor.grid.value;
            disposition.grid = value;
        })

        editor.move.addEventListener("mousedown", function (event) {
            const editingRect = editing.getBoundingClientRect();
            const editorRect = editor.root.getBoundingClientRect();
            moving = {
                editingX: event.clientX - editingRect.left,
                editingY: event.clientY - editingRect.top,
                editorX: event.clientX - editorRect.left,
                editorY: event.clientY - editorRect.top
            };
            document.body.style.userSelect = "none";
        });
        document.addEventListener("mousemove", event => {
            if (moving) {
                const x = grid(event.clientX - moving.editingX);
                const y = grid(event.clientY - moving.editingY);
                disposition.elements[editing.id].x = x;
                disposition.elements[editing.id].y = y;
                anabeeb.position(editing, x, y);

                anabeeb.position(editor.root, event.clientX - moving.editorX, event.clientY - moving.editorY);
            }
        });
        document.addEventListener("mouseup", event => {
            if (moving) {
                moving = undefined;
                document.body.style.userSelect = "";
            }
        });

        editor.bind.addEventListener("click", function () {
            if (binding) {
                editor.root.classList.remove('binding');
                binding = false;

                bindConfirm(editing);
            } else {
                editor.root.classList.add('binding');
                binding = true;

                bindStart(editing);
            }
        });

        editor.download.addEventListener("click", function () {
            const json = `window.disposition =  ${JSON.stringify(disposition)}`;
            const blob = new Blob([json], {type: "text/json"});

            editor.download.href = URL.createObjectURL(blob);
            editor.download.download = "disposition.js";
            setTimeout(() => URL.revokeObjectURL(editor.download.href), 2000);

            hideEditor();
        });

        editor.remove.addEventListener("click", function () {
            editing.remove();
            disposition.elements[editing.id] = undefined;

            hideEditor();
        });

        editor.add.addEventListener("click", function () {
            const ids = Array.from(editor.elements.selectedOptions).map(o => o.value);
            var { left, top } = editor.root.getBoundingClientRect();

            ids.forEach(id => {
                const type = elements[id].type;
                const templates = getTemplates(type);
                if (templates?.length) {
                    const element = {
                        label: id,
                        x: left,
                        y: top,
                        template: templates[0]
                    };
                    disposition.elements[id] = element;
                    anabeeb.added(id, element);
                    initElement(id, element);
                } else {
                    console.error(`no template found for element ${type}`)
                }

                left += 50;
                top += 50;
            });

            hideEditor();
        });

        document.addEventListener("contextmenu", (event) => {
            event.preventDefault();
            event.stopImmediatePropagation();

            showEditor(undefined, event.pageX, event.pageY);
        });
        document.addEventListener("mousedown", () => {
            hideEditor();
        });
    }

    function hideEditor() {
        if (editing) {
            editing.classList.remove("editing");
            editing = undefined;
        }
        if (binding) {
            editor.root.classList.remove('binding');
            bindCancel(editing)
            binding = false;
        }

        editor.root.close();
    }

    function showEditor(node, left, top) {
        hideEditor();

        editing = node;
        if (editing) {
            editing.classList.add("editing");

            editor.id.hidden = false;
            editor.id.textContent = editing.id;

            editor.label.hidden = false;
            editor.label.value = disposition.elements[editing.id].label;

            editor.template.hidden = false;
            editor.template.replaceChildren(...getTemplates(elements[editing.id].type)
                .map((template) => new Option(template, template))
            );
            editor.template.value = disposition.elements[editing.id].template;
            editor.scale.hidden = false;
            editor.scale.value = disposition.elements[editing.id].scale || 1.0;

            editor.uri.hidden = true;
            editor.grid.hidden = true;
            editor.elements.hidden = true;

            editor.move.hidden = false;
            editor.bind.hidden = false;
            editor.remove.hidden = false;
            editor.download.hidden = true;
            editor.add.hidden = true;
        } else {
            editor.id.hidden = true;
            editor.label.hidden = true;

            editor.template.hidden = true;
            editor.scale.hidden = true;

            editor.uri.hidden = false;
            editor.uri.value = disposition.uri;
            editor.grid.hidden = false;
            editor.grid.value = disposition.grid;
            editor.elements.hidden = false;
            editor.elements.replaceChildren(
                ...Object.entries(elements)
                    .map(([id, element]) => new Option(id, id))
            );

            editor.move.hidden = true;
            editor.bind.hidden = true;
            editor.remove.hidden = true;
            editor.download.hidden = false;
            editor.add.hidden = false;
        }

        anabeeb.position(editor.root, left, top);
        editor.root.show();
    }

    function initElement(id, element) {
        const node = document.getElementById(id);

        node.title = id;
        node.addEventListener("contextmenu", (event) => {
            event.preventDefault();
            event.stopImmediatePropagation();

            showEditor(node, event.pageX, event.pageY);
        });
    }

    window.addEventListener('load', () => {
        initEditor();

        anabeeb.get("/element").then(json => {
            elements = json.elements;
        });

        Object.entries(disposition.elements).forEach(([id, element]) => initElement(id, element));
    });
})();