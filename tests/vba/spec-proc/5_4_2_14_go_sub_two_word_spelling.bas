' INV: vbal.5.4.2.14
' SOURCE: spec ms-vbal 5.4.2.14 GoSub may be written as two words
' XFAIL: wrong pending
' EXPECT: sub
' EXPECT: back
Public Sub Main()
    Go Sub Work
    Debug.Print "back"
    Exit Sub
Work:
    Debug.Print "sub"
    Return
End Sub
