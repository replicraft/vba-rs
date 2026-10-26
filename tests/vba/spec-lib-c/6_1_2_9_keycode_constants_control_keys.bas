' INV: vbal.6.1.2.9
' SOURCE: spec ms-vbal 6.1.2.9 KeyCodeConstants control and navigation keys
' EXPECT: 1 2 3 4 8 9 12 13 16 17 18 19 20 27 32
' EXPECT: 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 144
Public Sub Main()
    Debug.Print vbKeyLButton & " " & vbKeyRButton & " " & vbKeyCancel & " " & vbKeyMButton & " " & vbKeyBack & " " & vbKeyTab & " " & vbKeyClear & " " & vbKeyReturn & " " & vbKeyShift & " " & vbKeyControl & " " & vbKeyMenu & " " & vbKeyPause & " " & vbKeyCapital & " " & vbKeyEscape & " " & vbKeySpace
    Debug.Print vbKeyPageUp & " " & vbKeyPageDown & " " & vbKeyEnd & " " & vbKeyHome & " " & vbKeyLeft & " " & vbKeyUp & " " & vbKeyRight & " " & vbKeyDown & " " & vbKeySelect & " " & vbKeyPrint & " " & vbKeyExecute & " " & vbKeySnapshot & " " & vbKeyInsert & " " & vbKeyDelete & " " & vbKeyHelp & " " & vbKeyNumlock
End Sub
