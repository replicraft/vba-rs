' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 data expression must not have a named class type
' ERROR: load
' GRANT: file-read
' GRANT: file-write
'#MODULE C1 class
Public x As Long
'#MODULE Module1 standard
Public Sub Main()
    Dim c As C1
    Open "/g.bin" For Binary As #1
    Put #1, 1, c
End Sub
