' INV: stmt.if-then-else, lit.null
' SOURCE: spec ms-vbal 5.6.16.3 Boolean expressions Null is False
' EXPECT: f
Public Sub Main()
    If Null Then Debug.Print "t" Else Debug.Print "f"
End Sub
