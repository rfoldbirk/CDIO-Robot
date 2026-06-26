from ultralytics import YOLO
import json
import cv2;
model = YOLO("yolo26m.pt")


# Open webcam
cap = cv2.VideoCapture(0)
if not cap.isOpened():
    print("Could not open camera")
else:
    print("Camera opened successfully")

cap = cv2.VideoCapture(0)


results = model("image.jpg")
detections = []

for box in results[0].boxes:
    class_id = int(box.cls[0])
    detections.append({
        "class": results[0].names[class_id],
        "confidence": float(box.conf[0]),
        "bbox": box.xyxy[0].tolist()
    })

with open("detections.json", "w") as f:
    json.dump(detections, f, indent=4)

print("Saved detections.json")
