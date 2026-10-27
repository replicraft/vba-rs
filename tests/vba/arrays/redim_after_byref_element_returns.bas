' INV: stmt.redim
' SOURCE: derived the lock on an array lasts only while a ByRef element argument is in use (MS-VBAL 5.4.3.3); resizing the whole array through a ByRef array parameter is allowed
' EXPECT: 5
' EXPECT: 7
' EXPECT: 9
Dim g() As Long
Private Sub Touch(ByRef x As Long)
    x = 5
End Sub
Private Sub Grow(ByRef a() As Long)
    ReDim a(9)
End Sub
Public Sub Main()
    ReDim g(3)
    Touch g(1)
    Debug.Print CStr(g(1))
    ReDim Preserve g(6)
    g(6) = 7
    Debug.Print CStr(g(6))
    Grow g
    Debug.Print CStr(UBound(g))
End Sub
