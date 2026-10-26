' INV: stmt.property-get, stmt.property-let
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/property-get-statement
' EXPECT:  9  0 
'#MODULE Grid class
Private cells(1 To 3) As Long
Public Property Get Cell(ByVal i As Long) As Long
    Cell = cells(i)
End Property
Public Property Let Cell(ByVal i As Long, ByVal v As Long)
    cells(i) = v
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim g As New Grid
    g.Cell(2) = 9
    Debug.Print g.Cell(2); g.Cell(1)
End Sub
