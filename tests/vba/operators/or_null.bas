' INV: op.or, lit.null
' SOURCE: derived Null Or True is True; Null Or False is Null
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Null Or True)
    Debug.Print IsNull(Null Or False)
End Sub
