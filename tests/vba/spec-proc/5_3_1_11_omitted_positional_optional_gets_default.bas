' INV: vbal.5.3.1.11, decl.optional
' SOURCE: spec ms-vbal 5.3.1.11 positional argument with omitted value for an optional parameter takes its default
' EXPECT: 1,20,3
Sub S(a As Long, Optional b As Long = 20, Optional c As Long = 30)
    Debug.Print CStr(a) & "," & CStr(b) & "," & CStr(c)
End Sub
Public Sub Main()
    S 1, , 3
End Sub
