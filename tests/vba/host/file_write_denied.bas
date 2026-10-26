' INV: stmt.open, obj.err
' SOURCE: derived deny by default: no FileWrite grant is error 70 Permission denied
' ERROR: 70
Public Sub Main()
    Open "/denied.txt" For Output As #1
End Sub
