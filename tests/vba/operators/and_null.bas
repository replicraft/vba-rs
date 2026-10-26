' INV: op.and, lit.null
' SOURCE: derived Null And False is False; Null And True is Null (three-valued logic)
' EXPECT: False
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Null And False)
    Debug.Print IsNull(Null And True)
End Sub
