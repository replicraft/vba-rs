' INV: stmt.function, decl.scope
' SOURCE: spec ms-vbal 5.6.10 other procedural module namespace tier, 5.6.12 module member access
' EXPECT:  42 
' EXPECT:  8 
'#MODULE Helper standard
Public Function Twice(n As Long) As Long
    Twice = n * 2
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print Twice(21)
    Debug.Print Helper.Twice(4)
End Sub
