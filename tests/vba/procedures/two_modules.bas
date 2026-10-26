' INV: stmt.function, decl.scope
' SOURCE: derived public procedures are visible across modules
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
