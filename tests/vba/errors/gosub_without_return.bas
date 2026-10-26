' INV: stmt.gosub-return
' SOURCE: spec ms-vbal 5.4.2.15 Return statement with an empty GoSub resumption list is error 3
' ERROR: 3
Public Sub Main()
    Return
End Sub
