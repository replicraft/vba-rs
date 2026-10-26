Attribute VB_Name = "Geometry"
Option Explicit
Option Compare Text

Public Const PI As Double = 3.14159265358979
Private Const MAX_SHAPES = 16

Public Enum ShapeKind
    skCircle = 1
    skSquare
    skTriangle = skSquare + 5
End Enum

Public Type Point2D
    X As Double
    Y As Double
End Type

Private Type Bounds
    TopLeft As Point2D
    BottomRight As Point2D
    Tags(1 To 3) As String
End Type

Private shapes(1 To MAX_SHAPES) As IShape
Private shapeCount As Long
Private lastError As String

Public Function Distance(a As Point2D, b As Point2D) As Double
    Dim dx As Double, dy As Double
    dx = a.X - b.X
    dy = a.Y - b.Y
    Distance = Sqr(dx * dx + dy * dy)
End Function

Public Sub AddShape(ByVal s As IShape)
    If shapeCount >= MAX_SHAPES Then
        Err.Raise vbObjectError + 1, "Geometry", "too many shapes"
    End If
    shapeCount = shapeCount + 1
    Set shapes(shapeCount) = s
End Sub

Public Function TotalArea() As Double
    Dim i As Long, sum As Double
    For i = 1 To shapeCount
        sum = sum + shapes(i).Area
    Next i
    TotalArea = sum
End Function

Public Function Describe(kind As ShapeKind) As String
    Select Case kind
        Case skCircle
            Describe = "circle"
        Case skSquare, skTriangle
            Describe = "polygon"
        Case Else
            Describe = "unknown (" & CStr(kind) & ")"
    End Select
End Function

Public Function SafeDivide(ByVal a As Double, ByVal b As Double, Optional ByVal fallback As Double = 0) As Double
    On Error GoTo Failed
    SafeDivide = a / b
    Exit Function
Failed:
    lastError = Err.Description
    SafeDivide = fallback
    Resume Next
End Function

Public Function Join3(ParamArray parts() As Variant) As String
    Dim v As Variant, r As String
    For Each v In parts
        r = r & CStr(v) & ","
    Next v
    Join3 = r
End Function

Public Sub Grow(ByRef values() As Long, ByVal extra As Long)
    ReDim Preserve values(LBound(values) To UBound(values) + extra)
End Sub

Public Function Counter() As Long
    Static n As Long
    n = n + 1
    Counter = n
End Function

Public Sub Demo()
    Dim p As Point2D, q As Point2D
    Dim box As Bounds
    Dim c As Collection
    Dim arr() As Long
    Dim k As Variant
    p.X = 1: p.Y = 2
    q.X = 4: q.Y = 6
    box.TopLeft = p
    box.Tags(2) = "tag"
    Set c = New Collection
    c.Add "one"
    c.Add "two", "k2"
    ReDim arr(0 To 3)
    Grow arr, 2
    With box.BottomRight
        .X = Distance(p, q)
        .Y = PI
    End With
    Do While shapeCount < 3
        shapeCount = shapeCount + 1
    Loop
    For Each k In c
        Debug.Print k
    Next k
    Debug.Print Describe(skCircle), TotalArea(), SafeDivide(1, 0, -1)
End Sub
