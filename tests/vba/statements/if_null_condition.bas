' INV: stmt.if-then-else, lit.null
' SOURCE: derived a Null condition is treated as False
' EXPECT: f
Public Sub Main()
    If Null Then Debug.Print "t" Else Debug.Print "f"
End Sub
