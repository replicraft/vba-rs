' INV: vbal.5.4.2.15, stmt.gosub-return, stmt.on-error
' SOURCE: spec ms-vbal 5.4.2.15 Return statement with an empty GoSub resumption list is error 3, which On Error Resume Next skips
' EXPECT:  3 
' EXPECT: after
Public Sub Main()
    On Error Resume Next
    Return
    Debug.Print Err.Number
    Debug.Print "after"
End Sub
