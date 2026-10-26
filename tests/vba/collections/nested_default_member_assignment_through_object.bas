' INV: expr.default-member
' SOURCE: derived a Variant holding an object followed by an index calls the default member, whose object result receives the next default-member assignment
' EXPECT:  42 
'#MODULE Row class
Private cells(5) As Long
Public Property Get Item(ByVal i As Long) As Long
Attribute Item.VB_UserMemId = 0
    Item = cells(i)
End Property
Public Property Let Item(ByVal i As Long, ByVal x As Long)
    cells(i) = x
End Property
'#MODULE Grid class
Private r As Row
Private Sub Class_Initialize()
    Set r = New Row
End Sub
Public Property Get Item(ByVal i As Long) As Row
Attribute Item.VB_UserMemId = 0
    Set Item = r
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim g As Variant
    Set g = New Grid
    g(1)(2) = 42
    Debug.Print g(1)(2)
End Sub
