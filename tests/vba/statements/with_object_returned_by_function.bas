' INV: stmt.with, expr.with-member
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/with-statement
' EXPECT:  7 
'#MODULE Box class
Public V As Long
'#MODULE Module1 standard
Dim keep As New Box
Function GetBox() As Box
    Set GetBox = keep
End Function
Public Sub Main()
    With GetBox()
        .V = 7
    End With
    Debug.Print keep.V
End Sub
