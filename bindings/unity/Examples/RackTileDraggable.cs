using UnityEngine;
using UnityEngine.EventSystems;
using UnityEngine.UI;

namespace TileTangle.Examples
{
    public class RackTileDraggable : MonoBehaviour, IBeginDragHandler, IDragHandler, IEndDragHandler
    {
        public string KindId = "";
        public string Symbol = "";
        public int Score = 0;

        private GameObject dragPreview;
        private Canvas canvas;

        void Awake()
        {
            canvas = FindObjectOfType<Canvas>();
        }

        public void OnBeginDrag(PointerEventData eventData)
        {
            if (canvas == null) return;
            dragPreview = new GameObject("DragPreview", typeof(RectTransform), typeof(CanvasGroup), typeof(Image));
            dragPreview.transform.SetParent(canvas.transform, false);
            var img = dragPreview.GetComponent<Image>();
            img.color = new Color(1f, 1f, 0.6f, 0.85f);
            var txtGo = new GameObject("Text", typeof(RectTransform), typeof(Text));
            txtGo.transform.SetParent(dragPreview.transform, false);
            var txt = txtGo.GetComponent<Text>();
            txt.font = Resources.GetBuiltinResource<Font>("Arial.ttf");
            txt.alignment = TextAnchor.MiddleCenter;
            txt.color = Color.black;
            txt.text = $"{Symbol}\n{Score}";
            var rt = dragPreview.GetComponent<RectTransform>();
            rt.sizeDelta = new Vector2(64, 64);
            dragPreview.GetComponent<CanvasGroup>().blocksRaycasts = false;
            UpdatePreviewPosition(eventData);
        }

        public void OnDrag(PointerEventData eventData)
        {
            UpdatePreviewPosition(eventData);
        }

        public void OnEndDrag(PointerEventData eventData)
        {
            if (dragPreview != null)
            {
                GameObject.Destroy(dragPreview);
                dragPreview = null;
            }
        }

        private void UpdatePreviewPosition(PointerEventData eventData)
        {
            if (dragPreview == null) return;
            RectTransformUtility.ScreenPointToLocalPointInRectangle(
                canvas.transform as RectTransform, eventData.position, eventData.pressEventCamera, out var pos);
            (dragPreview.transform as RectTransform).anchoredPosition = pos;
        }
    }
}

