' INV: vbal.5.3.1.5, decl.optional, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.5 parameter mechanism may precede or follow Optional
' EXPECT: 1
' EXPECT: x=1 y=2
' EXPECT: 5,6
Sub S(ByVal Optional a As Long = 5, Optional ByVal b As Long = 6)
    Debug.Print CStr(a)
    a = 99
    b = 98
End Sub
Sub T(Optional ByVal a As Long = 5, ByVal Optional b As Long = 6)
    Debug.Print CStr(a) & "," & CStr(b)
End Sub
Public Sub Main()
    Dim x As Long, y As Long
    x = 1: y = 2
    S x, y
    Debug.Print "x=" & CStr(x) & " y=" & CStr(y)
    T
End Sub
