using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;

namespace TileTangle.Examples
{
    public class BoardCellDropTarget : MonoBehaviour, IDropHandler, IPointerEnterHandler, IPointerExitHandler
    {
        public int X;
        public int Y;
        public BoardDemo Board;

        private Image bg;

        void Awake()
        {
            bg = GetComponent<Image>();
        }

        public void OnDrop(PointerEventData eventData)
        {
            if (eventData.pointerDrag == null) return;
            var drag = eventData.pointerDrag.GetComponent<RackTileDraggable>();
            if (drag == null) return;
            if (Board != null)
            {
                Board.AddStagedPlacement(X, Y, drag.KindId);
            }
        }

        public void OnPointerEnter(PointerEventData eventData)
        {
            if (bg != null) bg.color = new Color(0.85f, 1f, 0.85f, 1f);
        }

        public void OnPointerExit(PointerEventData eventData)
        {
            if (bg != null) bg.color = new Color(0.92f, 0.92f, 0.92f, 1f);
        }
    }
}

