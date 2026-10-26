' INV: vbal.5.3.1.2
' SOURCE: spec ms-vbal 5.3.1.2 initial Static makes all body variables module extent
' EXPECT: a=1,b=1
' EXPECT: a=2,b=2
Static Sub Bump()
    Dim a As Long
    Dim b As Long
    a = a + 1
    b = b + 1
    Debug.Print "a=" & CStr(a) & ",b=" & CStr(b)
End Sub
Public Sub Main()
    Bump
    Bump
End Sub
