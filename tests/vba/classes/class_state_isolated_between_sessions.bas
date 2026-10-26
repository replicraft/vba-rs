' INV: decl.scope
' SOURCE: spec ms-vbal 2.5.1 automatic object instantiation, 2.3 variable default values
' EXPECT:  1 
'#MODULE Counter class
Public N As Long
'#MODULE Module1 standard
Dim c As New Counter
Public Sub Main()
    c.N = c.N + 1
    Debug.Print c.N
End Sub
