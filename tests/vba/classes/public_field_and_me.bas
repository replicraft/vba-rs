' INV: decl.scope
' SOURCE: derived Me refers to the current instance
' EXPECT: x
' EXPECT: True
'#MODULE Node class
Public Name As String
Public Function Self() As Node
    Set Self = Me
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim n As New Node
    n.Name = "x"
    Debug.Print n.Self.Name
    Debug.Print n.Self Is n
End Sub
