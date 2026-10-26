' INV: vbal.5.4.5.5, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.5 Unlock on a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Unlock #9
End Sub
