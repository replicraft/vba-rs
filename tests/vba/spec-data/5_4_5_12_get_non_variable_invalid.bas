' INV: vbal.5.4.5.12, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 target of Get must be classified as a variable
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Open "/g.bin" For Binary As #1
    Get #1, 1, 5
End Sub
