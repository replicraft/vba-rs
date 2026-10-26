' INV: vbal.5.4.5.4, stmt.lock-unlock
' SOURCE: spec ms-vbal 5.4.5.4 Lock on a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Lock #9
End Sub
