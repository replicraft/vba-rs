' INV: expr.implicit-conv
' SOURCE: derived a number assigned to a String converts to text
' EXPECT: 12.5!
Public Sub Main()
    Dim s As String
    s = 12.5
    Debug.Print s & "!"
End Sub
