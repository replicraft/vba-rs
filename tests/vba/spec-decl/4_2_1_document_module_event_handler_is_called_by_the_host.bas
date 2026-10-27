' INV: vbal.4.2.1, decl.default-instance
' SOURCE: derived 4.2.1 a document module is extended by its host: an event the host raises reaches the handler declared in the module, which runs on the module's own instance (Me)
' METHOD: by hand; RAISE delivers Workbook_Open after the entry procedure
' RAISE: Workbook Open
' EXPECT: main
' EXPECT: opened ThisWorkbook
'#MODULE ThisWorkbook document
Private Sub Workbook_Open()
    Debug.Print "opened " & TypeName(Me)
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Debug.Print "main"
End Sub
