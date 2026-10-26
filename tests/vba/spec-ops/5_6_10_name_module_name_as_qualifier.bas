' INV: vbal.5.6.10, decl.scope
' SOURCE: spec ms-vbal 5.6.10 a procedural module name resolves in the project tier and qualifies members
' EXPECT: 12
' EXPECT: 5
' EXPECT: 9
'#MODULE Util standard
Public Const K As Long = 5
Public Counter As Long
Public Function Twice(ByVal n As Long) As Long
    Twice = n * 2
End Function
'#MODULE Module1 standard
Public Sub Main()
    Util.Counter = 9
    Debug.Print CStr(Util.Twice(6))
    Debug.Print CStr(Util.K)
    Debug.Print CStr(Util.Counter)
End Sub
