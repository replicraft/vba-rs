' INV: vbal.4.1
' SOURCE: derived 4.1 module names are the qualifiers of a project; two Public procedures of one name in different modules are told apart by them, and an unqualified use is ambiguous
' METHOD: by hand
' EXPECT: AB
'#MODULE A standard
Public Function Where() As String
    Where = "A"
End Function
'#MODULE B standard
Public Function Where() As String
    Where = "B"
End Function
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print A.Where & B.Where
End Sub
