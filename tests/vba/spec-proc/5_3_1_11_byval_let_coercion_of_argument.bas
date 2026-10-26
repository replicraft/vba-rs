' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByVal argument data value is Let-assigned to the parameter's local
' EXPECT: 3
' EXPECT: 42
Sub S(ByVal n As Long, ByVal t As Long)
    Debug.Print CStr(n)
    Debug.Print CStr(t)
End Sub
Public Sub Main()
    S 2.6, "42"
End Sub
