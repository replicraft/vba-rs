' INV: vbal.5.4.2.15
' SOURCE: spec ms-vbal 5.4.2.15 Return with an empty resumption list raises error 3
' EXPECT: before
' ERROR: 3
Public Sub Main()
    Debug.Print "before"
    Return
End Sub
