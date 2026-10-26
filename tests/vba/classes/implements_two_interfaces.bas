' INV: stmt.implements
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/implements-statement
' EXPECT: ab
'#MODULE IA class
Public Function A() As String
End Function
'#MODULE IB class
Public Function B() As String
End Function
'#MODULE Both class
Implements IA
Implements IB
Private Function IA_A() As String
    IA_A = "a"
End Function
Private Function IB_B() As String
    IB_B = "b"
End Function
'#MODULE Module1 standard
Public Sub Main()
    Dim x As New Both, ia As IA, ib As IB
    Set ia = x: Set ib = x
    Debug.Print ia.A() & ib.B()
End Sub
