' INV: vbal.5.3.1.5, decl.optional
' SOURCE: spec ms-vbal 5.3.1.5 default values of omitted optional parameters
' EXPECT: n=0 s=[] d=7 o=True
Sub S(Optional n As Long, Optional s As String, Optional d As Long = 7, Optional o As Object = Nothing)
    Debug.Print "n=" & CStr(n) & " s=[" & s & "] d=" & CStr(d) & " o=" & CStr(o Is Nothing)
End Sub
Public Sub Main()
    S
End Sub
