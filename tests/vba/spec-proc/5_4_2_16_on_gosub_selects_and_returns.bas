' INV: vbal.5.4.2.16
' SOURCE: spec ms-vbal 5.4.2.16 On n GoSub calls the n-th label and Return resumes after the statement; out of range falls through
' EXPECT: two
' EXPECT: resumed
' EXPECT: resumed
' EXPECT: end
Public Sub Main()
    On 2 GoSub A, B
    Debug.Print "resumed"
    On 0 GoSub A, B
    Debug.Print "resumed"
    On 3 GoSub A, B
    Debug.Print "end"
    Exit Sub
A:
    Debug.Print "one"
    Return
B:
    Debug.Print "two"
    Return
End Sub
