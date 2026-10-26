' INV: decl.default-instance
' SOURCE: derived a host event is delivered to a handler on a document module's default instance
' EXPECT: main
' EXPECT: opened
' RAISE: Workbook Open
'#MODULE ThisWorkbook document
Private Sub Workbook_Open()
    Debug.Print "opened"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "main"
End Sub
