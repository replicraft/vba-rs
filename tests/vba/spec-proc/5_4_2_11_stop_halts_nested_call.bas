' INV: vbal.5.4.2.11
' SOURCE: derived ms-vbal 5.4.2.11 Stop halts execution of the program, also from inside a called procedure; nothing after it runs
' EXPECT: before
' EXPECT: inner
Private Sub Inner()
    Debug.Print "inner"
    Stop
    Debug.Print "not reached"
End Sub
Public Sub Main()
    Debug.Print "before"
    Inner
    Debug.Print "after"
End Sub
