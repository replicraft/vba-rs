' INV: vbal.5.4.2.15
' SOURCE: spec ms-vbal 5.4.2.15 each Return consumes one entry; a further Return raises error 3
' EXPECT: sub
' EXPECT: after-gosub
' ERROR: 3
Public Sub Main()
    GoSub L
    Debug.Print "after-gosub"
    Return
L:
    Debug.Print "sub"
    Return
End Sub
