' INV: op.concat, lit.null
' SOURCE: derived Null is treated as a zero-length string by &
' EXPECT: x
' EXPECT: a
' EXPECT: True
Public Sub Main()
    Debug.Print Null & "x"
    Debug.Print "a" & Null
    Debug.Print IsNull(Null & Null)
End Sub
