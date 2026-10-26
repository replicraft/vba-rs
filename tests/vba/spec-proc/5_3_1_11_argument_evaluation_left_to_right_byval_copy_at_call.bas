' INV: vbal.5.3.1.11
' SOURCE: spec ms-vbal 5.3.1.11 ByVal parameter holds the argument's value as of invocation
' EXPECT: a=1 b=100
Dim g As Long
Sub S(ByVal a As Long, ByVal b As Long)
    g = 100
    Debug.Print "a=" & CStr(a) & " b=" & CStr(b)
End Sub
Public Sub Main()
    g = 1
    S g, g + 99
End Sub
