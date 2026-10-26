' INV: func.timer
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/timer-function
' EXPECT: True
Public Sub Main()
    Debug.Print Timer >= 0 And Timer < 86400
End Sub
