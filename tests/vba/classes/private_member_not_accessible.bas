' INV: stmt.private
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/private-statement
' ERROR: load
'#MODULE Box class
Private V As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim b As New Box
    b.V = 1
End Sub
