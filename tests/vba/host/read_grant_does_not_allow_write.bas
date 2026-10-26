' INV: stmt.open
' SOURCE: derived capabilities are independent
' ERROR: 70
' GRANT: file-read
Public Sub Main()
    Open "/x.txt" For Output As #1
End Sub
