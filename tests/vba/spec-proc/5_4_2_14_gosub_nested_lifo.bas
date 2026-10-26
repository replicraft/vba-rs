' INV: vbal.5.4.2.14, vbal.5.4.2.15
' SOURCE: spec ms-vbal 5.4.2.14 GoSub resumption list is last-in first-out
' EXPECT: outer-in
' EXPECT: inner
' EXPECT: outer-out
' EXPECT: main-after
Public Sub Main()
    GoSub Outer
    Debug.Print "main-after"
    Exit Sub
Outer:
    Debug.Print "outer-in"
    GoSub Inner
    Debug.Print "outer-out"
    Return
Inner:
    Debug.Print "inner"
    Return
End Sub
