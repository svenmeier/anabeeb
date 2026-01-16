(function() {

    let editor;

    let editing = undefined;
    let binding = false;
    let moving = undefined;

    let elements = undefined;

    function grid(n) {
        return Math.round(n / disposition.grid) * disposition.grid;
    }

    function bindStart(id) {
        anabeeb.post(`/binding/${id}/start`);
    }

    function bindCancel(id) {
        anabeeb.post(`/binding/${id}/cancel`);
    }

    function bindConfirm(id) {
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
            disposition.elements[editing].label = value;
            document.getElementById(editing).querySelector('label').textContent = value;
        });

        editor.template.addEventListener('change', () => {
            const value = editor.template.value;
            const element = disposition.elements[editing];
            element.template = value;
            anabeeb.added(editing, element);
            initElement(editing);
        })
        editor.scale.addEventListener('change', () => {
            const value = editor.scale.value;
            disposition.elements[editing].scale = value;
            document.getElementById(editing).style.scale = value;
        })

        editor.uri.addEventListener('change', () => {
            disposition.uri = editor.uri.value;
        })
        editor.grid.addEventListener('change', () => {
            disposition.grid = editor.grid.value;
        })

        editor.move.addEventListener("mousedown", function (event) {
            const editingRect = document.getElementById(editing).getBoundingClientRect();
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
                disposition.elements[editing].x = x;
                disposition.elements[editing].y = y;
                anabeeb.position(document.getElementById(editing), x, y);

                anabeeb.position(editor.root, event.clientX - moving.editorX, event.clientY - moving.editorY);
            }
        });
        document.addEventListener("mouseup", () => {
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
            const json = `${JSON.stringify(disposition)}`;
            const blob = new Blob([json], {type: "text/json"});

            editor.download.href = URL.createObjectURL(blob);
            editor.download.download = "disposition.js";
            setTimeout(() => URL.revokeObjectURL(editor.download.href), 2000);

            hideEditor();
        });

        editor.remove.addEventListener("click", function () {
            document.getElementById(editing).remove();
            disposition.elements[editing] = undefined;

            hideEditor();
        });

        editor.add.addEventListener("click", function () {
            const ids = Array.from(editor.elements.selectedOptions).map(o => o.value);
            let {left, top} = editor.root.getBoundingClientRect();
            left += window.scrollX;
            top += window.scrollY;

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

            if (ids.length === 1) {
                showEditor(ids[0]);
            } else {
                hideEditor();
            }
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
            if (binding) {
                editor.root.classList.remove('binding');
                bindCancel(editing)
                binding = false;
            }

            const node = document.getElementById(editing);
            node?.classList.remove("editing");
            editing = undefined;
        }

        editor.root.close();
    }

    function showEditor(id, left, top) {
        hideEditor();

        editing = id;
        if (editing) {
            const node = document.getElementById(id);
            node.classList.add("editing");
            const rect = node.getBoundingClientRect();
            left = rect.left + rect.width/2;
            top = rect.top + rect.height/2;

            editor.id.hidden = false;
            editor.id.textContent = id;

            editor.label.hidden = false;
            editor.label.value = disposition.elements[id].label;

            editor.template.hidden = false;
            editor.template.replaceChildren(...getTemplates(elements[id].type)
                .map((template) => new Option(template, template))
            );
            editor.template.value = disposition.elements[id].template;
            editor.scale.hidden = false;
            editor.scale.value = disposition.elements[id].scale || 1.0;

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
                    .map(([id, _]) => {
                        const option = new Option(id, id);
                        if (disposition.elements[id]) {
                            option.classList.add('marked');
                            option.addEventListener("dblclick", () => {
                                showEditor(id)
                            });
                        }
                        return option;
                    })
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

    function initElement(id) {
        const node = document.getElementById(id);

        node.title = id;
        node.addEventListener("contextmenu", (event) => {
            event.preventDefault();
            event.stopImmediatePropagation();

            showEditor(id);
        });
    }

    window.addEventListener('load', () => {
        initEditor();

        anabeeb.get("/element").then(json => {
            elements = json.elements;
        });

        Object.entries(disposition.elements).forEach(([id, _]) => initElement(id));
    });
})();