' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef Variant parameter binds to variables, elements, fields and UDT members of any declared type; the variable keeps its declared type
' EXPECT: 2 1
' EXPECT: 6
' EXPECT: 1
' EXPECT: 1
' EXPECT: 8 String
' EXPECT: True
' EXPECT: 16 24
' EXPECT: 2.5
Dim gl As Long
Type Pt
    X As Long
End Type
Sub Swap(ByRef a As Variant, ByRef b As Variant)
    Dim t As Variant
    t = a: a = b: b = t
End Sub
Sub Bump(ByRef v As Variant)
    v = v + 1
End Sub
Sub Drop(ByRef v As Variant)
    Set v = Nothing
End Sub
Function Twice(ByRef v As Variant) As Long
    v = v * 2
    Twice = v
End Function
Public Sub Main()
    Dim x As Long, y As Long
    x = 1: y = 2
    Swap x, y
    Debug.Print CStr(x) & " " & CStr(y)
    Dim arr(1) As Integer
    arr(0) = 5
    Bump arr(0)
    Debug.Print CStr(arr(0))
    Bump gl
    Debug.Print CStr(gl)
    Dim p As Pt
    Bump p.X
    Debug.Print CStr(p.X)
    Dim s As String
    s = "7"
    Bump s
    Debug.Print s & " " & TypeName(s)
    Dim c As Collection
    Set c = New Collection
    Drop c
    Debug.Print CStr(c Is Nothing)
    Dim n As Long, r As Long
    n = 4
    r = Twice(n) + Twice(n)
    Debug.Print CStr(n) & " " & CStr(r)
    Dim d As Double
    d = 1.5
    Bump d
    Debug.Print CStr(d)
End Sub
