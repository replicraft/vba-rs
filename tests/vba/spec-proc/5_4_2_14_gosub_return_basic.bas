' INV: vbal.5.4.2.14
' SOURCE: spec ms-vbal 5.4.2.14 GoSub branches to the label; Return resumes after the GoSub statement
' EXPECT: start
' EXPECT: sub
' EXPECT: back
' EXPECT: sub
' EXPECT: done
Public Sub Main()
    Debug.Print "start"
    GoSub Work
    Debug.Print "back"
    GoSub Work
    Debug.Print "done"
    Exit Sub
Work:
    Debug.Print "sub"
    Return
End Sub
