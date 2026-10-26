' INV: vbal.5.4.2.6, stmt.do-loop
' SOURCE: spec ms-vbal 5.4.2.6 position of the condition clause decides pre- or post-testing
' EXPECT: while-pre=0
' EXPECT: until-pre=0
' EXPECT: while-post=1
' EXPECT: until-post=1
' EXPECT: loop-until=3
Public Sub Main()
    Dim n As Long
    n = 0
    Do While False
        n = n + 1
    Loop
    Debug.Print "while-pre=" & CStr(n)
    n = 0
    Do Until True
        n = n + 1
    Loop
    Debug.Print "until-pre=" & CStr(n)
    n = 0
    Do
        n = n + 1
    Loop While False
    Debug.Print "while-post=" & CStr(n)
    n = 0
    Do
        n = n + 1
    Loop Until True
    Debug.Print "until-post=" & CStr(n)
    n = 0
    Do
        n = n + 1
    Loop Until n >= 3
    Debug.Print "loop-until=" & CStr(n)
End Sub
