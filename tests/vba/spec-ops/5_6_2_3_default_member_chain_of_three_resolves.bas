' INV: vbal.5.6.2.3, expr.default-member
' SOURCE: derived 5.6.2.3 allows an implementation-defined limit on default member recursion; a chain of three default members is far below any sensible limit and resolves to the leaf value
' METHOD: by hand: A's default returns B, B's default returns C, C's default returns the String "leaf"
' EXPECT: leaf
'#MODULE C class
Public Property Get Item() As String
Attribute Item.VB_UserMemId = 0
    Item = "leaf"
End Property
'#MODULE B class
Public Property Get Item() As C
Attribute Item.VB_UserMemId = 0
    Set Item = New C
End Property
'#MODULE A class
Public Property Get Item() As B
Attribute Item.VB_UserMemId = 0
    Set Item = New B
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim x As New A
    Dim s As String
    s = x
    Debug.Print s
End Sub
